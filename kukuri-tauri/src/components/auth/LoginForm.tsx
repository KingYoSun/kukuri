import { useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';
import { ArrowLeft, Eye, EyeOff } from 'lucide-react';
import { errorHandler } from '@/lib/errorHandler';

export function LoginForm() {
  const navigate = useNavigate();
  const { loginWithNsec } = useAuthStore();
  const [nsec, setNsec] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [saveToSecureStorage, setSaveToSecureStorage] = useState(true);
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!nsec.trim()) {
      toast.error('秘密鍵（nsec）を入力してください');
      return;
    }

    // NIP-19準拠のnsec形式チェック
    if (!nsec.startsWith('nsec1')) {
      toast.error('無効な形式です。nsec1で始まる秘密鍵を入力してください');
      return;
    }

    setIsLoading(true);
    try {
      await loginWithNsec(nsec, saveToSecureStorage);
      toast.success('ログインしました');
      await navigate({ to: '/' });
    } catch (error) {
      toast.error('ログインに失敗しました。秘密鍵を確認してください');
      errorHandler.log('Login failed', error, {
        context: 'LoginForm.handleSubmit',
      });
    } finally {
      setIsLoading(false);
    }
  };

  const handleBack = () => {
    navigate({ to: '/welcome' });
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <Card className="w-full max-w-md">
        <CardHeader>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleBack}
            className="w-fit -ml-2 mb-2"
          >
            <ArrowLeft className="w-4 h-4 mr-2" />
            戻る
          </Button>
          <CardTitle>ログイン</CardTitle>
          <CardDescription>
            既存のアカウントでログインします
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="nsec">秘密鍵（nsec）</Label>
              <div className="relative">
                <Input
                  id="nsec"
                  type={showPassword ? 'text' : 'password'}
                  value={nsec}
                  onChange={(e) => setNsec(e.target.value)}
                  placeholder="nsec1..."
                  className="pr-10"
                  autoComplete="off"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="absolute right-0 top-0 h-full px-3"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                nsec1で始まるNostr秘密鍵を入力してください
              </p>
            </div>
            
            <div className="flex items-center space-x-2">
              <Checkbox
                id="save-to-secure-storage"
                checked={saveToSecureStorage}
                onCheckedChange={(checked) => setSaveToSecureStorage(checked as boolean)}
              />
              <Label
                htmlFor="save-to-secure-storage"
                className="text-sm font-normal cursor-pointer"
              >
                アカウントを安全に保存して、次回から自動的にログインする
              </Label>
            </div>
            
            <div className="bg-warning/10 border border-warning rounded-md p-3">
              <p className="text-sm text-warning-foreground">
                ⚠️ 秘密鍵は絶対に他人に教えないでください
              </p>
            </div>

            <Button type="submit" className="w-full" disabled={isLoading}>
              {isLoading ? 'ログイン中...' : 'ログイン'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}