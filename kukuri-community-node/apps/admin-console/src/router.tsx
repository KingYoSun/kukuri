import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router';

import App from './App';
import { AuditPage } from './pages/AuditPage';
import { DashboardPage } from './pages/DashboardPage';
import { PoliciesPage } from './pages/PoliciesPage';
import { ServicesPage } from './pages/ServicesPage';
import { SubscriptionsPage } from './pages/SubscriptionsPage';

const rootRoute = createRootRoute({
  component: App
});

const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DashboardPage
});

const servicesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/services',
  component: ServicesPage
});

const subscriptionsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/subscriptions',
  component: SubscriptionsPage
});

const policiesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/policies',
  component: PoliciesPage
});

const auditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/audit',
  component: AuditPage
});

const routeTree = rootRoute.addChildren([
  dashboardRoute,
  servicesRoute,
  subscriptionsRoute,
  policiesRoute,
  auditRoute
]);

const basepath = import.meta.env.BASE_URL.replace(/\/$/, '');

export const router = createRouter({
  routeTree,
  basepath: basepath === '' ? '/' : basepath
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
