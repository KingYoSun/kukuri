import { createFileRoute } from '@tanstack/react-router';
import { ProfileSetup } from '@/components/auth/ProfileSetup';

export const Route = createFileRoute('/profile-setup')({
  component: ProfileSetup,
});
