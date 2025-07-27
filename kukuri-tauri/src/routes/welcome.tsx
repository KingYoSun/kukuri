import { createFileRoute } from '@tanstack/react-router';
import { WelcomeScreen } from '@/components/auth/WelcomeScreen';

export const Route = createFileRoute('/welcome')({
  component: WelcomeScreen,
});