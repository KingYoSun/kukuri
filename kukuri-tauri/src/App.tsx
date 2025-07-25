import "./App.css";
import { MainLayout } from "./components/layout/MainLayout";
import { Home } from "./pages/Home";
import { Toaster } from "sonner";

function App() {
  return (
    <>
      <MainLayout>
        <Home />
      </MainLayout>
      <Toaster />
    </>
  );
}

export default App;
