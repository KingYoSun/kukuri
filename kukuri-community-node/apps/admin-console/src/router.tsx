import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router';

import App from './App';
import { AccessControlPage } from './pages/AccessControlPage';
import { AuditPage } from './pages/AuditPage';
import { BootstrapPage } from './pages/BootstrapPage';
import { DashboardPage } from './pages/DashboardPage';
import { IndexPage } from './pages/IndexPage';
import { ModerationPage } from './pages/ModerationPage';
import { PoliciesPage } from './pages/PoliciesPage';
import { PrivacyDataPage } from './pages/PrivacyDataPage';
import { RelayPage } from './pages/RelayPage';
import { ServicesPage } from './pages/ServicesPage';
import { SubscriptionsPage } from './pages/SubscriptionsPage';
import { TrustPage } from './pages/TrustPage';

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

const relayRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/relay',
  component: RelayPage
});

const bootstrapRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/bootstrap',
  component: BootstrapPage
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

const privacyDataRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/privacy-data',
  component: PrivacyDataPage
});

const moderationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/moderation',
  component: ModerationPage
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/index',
  component: IndexPage
});

const auditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/audit',
  component: AuditPage
});

const trustRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/trust',
  component: TrustPage
});

const accessControlRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/access-control',
  component: AccessControlPage
});

const routeTree = rootRoute.addChildren([
  dashboardRoute,
  servicesRoute,
  relayRoute,
  bootstrapRoute,
  subscriptionsRoute,
  policiesRoute,
  privacyDataRoute,
  moderationRoute,
  indexRoute,
  accessControlRoute,
  auditRoute,
  trustRoute
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
