import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { DomeConnectionTopologyView, GameRoomView } from '@/lib/api';
import type { SupportedLocale } from '@/i18n';
import { connectionMap } from './DomeConnectionModel';

export function DomeConnectionMap({ topology, room, rooms, locale }: {
  topology: DomeConnectionTopologyView; room: GameRoomView; rooms: GameRoomView[]; locale: SupportedLocale;
}) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const map = useMemo(() => connectionMap(topology, room, rooms), [topology, room, rooms]);
  const radius = Math.max(1, ...map.nodes.flatMap(n => [Math.abs(n.x), Math.abs(n.z)]));
  const scale = 100 / radius;
  const points = new Map(map.nodes.map(n => [n.id, { x: 140 + n.x * scale, y: 130 + n.z * scale }]));
  return <figure className='connection-map'>
    <figcaption>{t('connections.map.title')}</figcaption>
    <svg viewBox='0 0 280 260' role='img' aria-label={t('connections.map.title')}>
      <text x='140' y='16' textAnchor='middle'>{t('connections.directions.north')} ↑</text>
      {map.edges.map(({ record }) => {
        const a = points.get(record.agreement.proposer.instance_id)!;
        const b = points.get(record.agreement.receiver.instance_id)!;
        return <line key={record.agreement.connection_id} x1={a.x} y1={a.y} x2={b.x} y2={b.y} className={record.status === 'draining' ? 'connection-edge-draining' : ''} />;
      })}
      {map.nodes.map((node, index) => {
        const p = points.get(node.id)!;
        return <g key={node.id} className={node.current ? 'connection-map-current' : ''}>
          <title>{node.room?.title ?? t('connections.map.unknown')}</title>
          <circle cx={p.x} cy={p.y} r='16' />
          <text x={p.x} y={p.y + 5} textAnchor='middle'>{index + 1}</text>
        </g>;
      })}
    </svg>
    <ol>{map.nodes.map(node => <li key={node.id}>{node.current && `${t('connections.map.here')}: `}{node.room?.title ?? t('connections.map.unknown')}</li>)}</ol>
    <ul className='connection-map-edges'>{map.edges.map(({ record }) => <li key={record.agreement.connection_id}>
      {map.nodes.find(n => n.id === record.agreement.proposer.instance_id)?.room?.title ?? t('connections.map.unknown')}
      {' — '}{t(`connections.directions.${record.agreement.proposer.direction}`)}{' → '}
      {map.nodes.find(n => n.id === record.agreement.receiver.instance_id)?.room?.title ?? t('connections.map.unknown')}
      {` (${t(`connections.status.${record.status}`)})`}
    </li>)}</ul>
  </figure>;
}
