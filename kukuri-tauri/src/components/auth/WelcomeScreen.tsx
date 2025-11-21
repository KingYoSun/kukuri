import { useNavigate } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

export function WelcomeScreen() {
  const navigate = useNavigate();
  const { generateNewKeypair } = useAuthStore();

  const handleCreateAccount = async () => {
    errorHandler.info('Starting account creation...', 'WelcomeScreen.handleCreateAccount');
    try {
      const result = await generateNewKeypair();
      const authState =
        typeof useAuthStore.getState === 'function' ? useAuthStore.getState() : null;
      errorHandler.info(
        'Keypair generated, navigating to profile setup',
        'WelcomeScreen.handleCreateAccount',
        {
          hasNsec: Boolean(result?.nsec),
          isAuthenticated: authState?.isLoggedIn ?? false,
          currentNpub: authState?.currentUser?.npub ?? null,
        },
      );
      await navigate({ to: '/profile-setup' });
      errorHandler.info(
        'Navigation to /profile-setup requested',
        'WelcomeScreen.handleCreateAccount',
      );
    } catch (error) {
      toast.error('アカウントの作成に失敗しました');
      errorHandler.log('Failed to create account', error, {
        context: 'WelcomeScreen.handleCreateAccount',
      });
    }
  };

  const handleLogin = () => {
    navigate({ to: '/login' });
  };

  return (
    <div
      className="flex items-center justify-center min-h-screen bg-background"
      data-testid="welcome-screen"
    >
      <Card className="w-full max-w-md">
        <CardHeader className="text-center space-y-4">
          <div className="mx-auto w-16 h-16 bg-primary rounded-full flex items-center justify-center">
            <span className="text-2xl font-bold text-primary-foreground">K</span>
          </div>
          <div>
            <CardTitle className="text-3xl font-bold">kukuriへようこそ</CardTitle>
            <CardDescription className="mt-2">
              分散型トピック中心ソーシャルアプリケーション
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2 text-sm text-muted-foreground">
            <p>・トピックベースのタイムラインで情報を共有</p>
            <p>・P2Pネットワークによる検閲耐性</p>
            <p>・Nostrプロトコルによる分散型アーキテクチャ</p>
          </div>
          <div className="space-y-3">
            <Button
              onClick={handleCreateAccount}
              className="w-full"
              size="lg"
              data-testid="welcome-create-account"
            >
              新規アカウント作成
            </Button>
            <Button
              onClick={handleLogin}
              variant="outline"
              className="w-full"
              size="lg"
              data-testid="welcome-login"
            >
              既存アカウントでログイン
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
