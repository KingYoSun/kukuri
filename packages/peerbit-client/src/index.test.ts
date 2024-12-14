import { MyDatabase, TextDocument } from "./index.js";
import { Peerbit } from "peerbit";
import { SearchRequest, Sort, SortDirection } from "@peerbit/document";
import { serialize, deserialize } from "@dao-xyz/borsh";
import { Program } from "@peerbit/program";
import { toBase64, fromBase64, randomBytes } from "@peerbit/crypto";
import { waitFor } from "@peerbit/time";

describe("suite", () => {
  let client: Peerbit;
  let client2: Peerbit;

  afterEach(async () => {
    await client?.stop();
    await client2?.stop();
  });

  it("start", async () => {
    client = await Peerbit.create({
      // More info about configs here https://github.com/libp2p/js-libp2p/blob/master/doc/GETTING_STARTED.md#configuring-libp2p
      // More info about configs here https://github.com/libp2p/js-libp2p/blob/master/doc/GETTING_STARTED.md#configuring-libp2p
      // You can provide a preconfigured libp2p client here, or omit it,
      // and it will created for you
      /* libp2p: {
				transports: [webSockets()],
				connectionEncryption: [noise()], // Make connections encrypted
			}, */
    });

    // you can also open the client with default config
    // client= await Peerbit.create({}) // will use weboSockets, tcp transport with noise encryption, and a new identitty will be created

    const db = await client.open(new MyDatabase());
    console.log(db.address.toString());
    expect(db.address.toString().length).toBeGreaterThan(0); // Some address like
  });

  it("adds 100 document and search for all of them", async () => {
    client = await Peerbit.create();

    const db = await client.open(new MyDatabase());
    console.log(db.address.toString());

    for (let i = 0; i < 100; i++) {
      await db.documents.put(new TextDocument("This is document #" + i));
    }
    const results = await db.documents.index.search(
      new SearchRequest({ query: [], fetch: 100 }),
      { local: true, remote: false },
    );
    expect(results).toHaveLength(100);
    console.log("First document:", (results[0] as TextDocument).text);

    // we can also query + sort
    const resultsSorted = await db.documents.index.iterate(
      new SearchRequest({
        query: [],
        sort: [new Sort({ direction: SortDirection.ASC, key: "text" })],
      }),
      { local: true, remote: false },
    );
    const [first, second, third] = (await resultsSorted.next(3)) as [
      TextDocument,
      TextDocument,
      TextDocument,
    ]; // cast BaseDocument to TextDocument (assume safe, else we can do instanceof checks before casting)
    console.log(
      `Sorted documents: first "${first.text}", second "${second.text}", third "${third.text}"`,
    );
  });

  it("save-load database from disc", async () => {
    const directory = "./tmp/test/1";

    // Cleanup from last run
    const fs = await import("fs");
    fs.existsSync(directory) && fs.rmSync(directory, { recursive: true }); // eslint-disable-line

    // In order to get a recoverable state we need to pass 'directory' param when creating client
    // this will ensure that we create a client that store content on disc rather than in RAM
    client = await Peerbit.create({
      // More info about configs here https://github.com/libp2p/js-libp2p/blob/master/doc/GETTING_STARTED.md#configuring-libp2p
      // You can provide a preconfigured libp2p client here, or omit it,
      // and it will created for you
      /* libp2p: {
				transports: [webSockets()],
				connectionEncryption: [noise()],
			},
			*/
      // Pass directory here
      // In this directory keys and data will be stored
      directory,
    });

    // Create a db as in the test before and add some documents
    let db = await client.open(new MyDatabase());
    const address = db.address.toString();
    for (let i = 0; i < 100; i++) {
      await db.documents.put(new TextDocument("This is document #" + i));
    }

    // Stop it (stops running processes like replication and networking).
    // In "real" world apps where a server crashes, this will not be called
    // and that is fine, everything is already written to disc.
    await client.stop();

    // reload client from same directory and see if data persists
    client = await Peerbit.create({
      // Same directory here
      directory,
    });

    // Open will automatically load previos state from disc
    db = await client.open<MyDatabase>(address);

    const results = await db.documents.index.search(
      new SearchRequest({ query: [], fetch: 100 }),
      { local: true, remote: false },
    ); // Only search locally
    expect(results).toHaveLength(100);
    console.log("First document:", (results[0] as TextDocument).text);
  });

  it("can create a database with same address everytime on open", async () => {
    // when you do client.open("/peerbit/abc123xyz"), the address will be converted into a CID which will be queried from peers
    // if you don't have the content it represents locally.
    // this can be great for online apps, but can be troublesome for apps that are mostly offline

    // In this test we are going too see that we can create a database with the same address everytime
    // by providing the "id" argument
    // so that you will not have to ask peers for database manifests if you are opening the database for the first time

    client = await Peerbit.create();

    const FIXED_DATABASE_ID = randomBytes(32);

    const db1 = await client.open(new MyDatabase({ id: FIXED_DATABASE_ID }));
    const address1 = db1.address;
    await db1.drop();

    const db2 = await client.open(new MyDatabase({ id: FIXED_DATABASE_ID }));
    const address2 = db2.address;

    expect(address1.toString()).toEqual(address2.toString());

    // The reason why the addresses become the same is because there are no "random" properties if you assign the id
    // Internally, when you do "client.open(new MyDatabase({ id: "some-id" }))", the database will be serialized and saved
    // and the serialized bytes of new MyDatabase({ id: "some-id" }) will determine the address of the database, through a hash function

    // when new MyDatabase({ id: SOME_FIXED_ID }) will be serialized, it will output exactly the same bytes, every time
    // This means that the hash of the serialized bytes will be the same
    // hence the address will be the same

    // this is useful when you don't want to manage an address, but you want to hardcode a constructor
    // or when you wan't to create a "local" first app, where you want to be able to load the database without having to ask peers
    // how to load specific database address

    // Below is a showcase how you also can manually serialize/deserialize the database manifest
    // (so you can share it with peers rather than the address if you want them to be able to load the database for the first time,
    // while beeing offline). Just from a byte array

    const bytes = serialize(db2); // Uint8Array, serialize our database "manifest"
    const base64 = toBase64(bytes);
    // you can send these 'bytes' or the base64 string to your peer, so that they can load the database address without beeing connect to anyone

    // Below is from the peer that just have seen the base64 and does not know what database it represents
    const deserializedProgram = deserialize(fromBase64(base64), Program); // We deserialize into "Program" (which could be any database)
    expect(deserializedProgram).toBeInstanceOf(MyDatabase); // If we do type checking we can see that it correctly deserialzied it into MyDatabas

    // we need to drop db2 because else the new open
    // will cause an exception as it will open the same address
    // Having two open database with the same address is currently not allowed
    await db2.drop();

    const db3 = await client.open(deserializedProgram as MyDatabase);
    expect(db3).toBeInstanceOf(MyDatabase);
  });

  it("can sync between peers", async () => {
    client = await Peerbit.create();
    client2 = await Peerbit.create();

    // Connect clients

    await client.dial(client2); // you can also do 'client.dial(client2.libp2p.getMultiaddrs())' to dial specific addresses

    const db1 = await client.open(new MyDatabase());
    await db1.documents.put(new TextDocument("Hello"));
    const db2 = await client2.open<MyDatabase>(db1.address); // role = 'replicator' by default

    // You can also open with role ObserverType, to not do any replication work
    // const db2 = await client2.open<MyDatabase>(db1.address, { role: 'observer' })
    await db2.documents.put(new TextDocument("World")); // role = 'replicator' by default

    // Check that both clients now have both documents
    // it documents will not sync be synced after 'await put' so we have to wait for them to arrive
    await waitFor(async () => (await db1.documents.index.getSize()) === 2); // Now synced!
    await waitFor(async () => (await db2.documents.index.getSize()) === 2); // Now synced!

    const results1 = await db1.documents.index.search(
      new SearchRequest({ query: [] }),
      { local: true, remote: false },
    );
    expect(results1.map((r) => (r as TextDocument).text).sort()).toEqual([
      "Hello",
      "World",
    ]);

    const results2 = await db1.documents.index.search(
      new SearchRequest({ query: [] }),
      { local: true, remote: false },
    );
    expect(results2.map((r) => (r as TextDocument).text).sort()).toEqual([
      "Hello",
      "World",
    ]);
  });
});
