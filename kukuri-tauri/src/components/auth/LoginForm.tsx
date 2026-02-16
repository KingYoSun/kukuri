import { useTranslation } from 'react-i18next';
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
import { useTheme } from '@/hooks/useTheme';

export function LoginForm() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { loginWithNsec, addAccount, isAuthenticated } = useAuthStore();
  useTheme(); // Apply theme to HTML element
  const [nsec, setNsec] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [saveToSecureStorage, setSaveToSecureStorage] = useState(true);
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!nsec.trim()) {
      toast.error(t('auth.enterNsec'));
      return;
    }

    // NIP-19準拠のnsec形式チェック
    if (!nsec.startsWith('nsec1')) {
      toast.error(t('auth.invalidNsecFormat'));
      return;
    }

    setIsLoading(true);
    try {
      if (isAuthenticated) {
        await addAccount(nsec);
        toast.success(t('auth.accountAddedSuccess'));
        await navigate({ to: '/' });
      } else {
        await loginWithNsec(nsec, saveToSecureStorage);
        toast.success(t('auth.loginSuccess'));
        await navigate({ to: '/' });
      }
    } catch (error) {
      toast.error(isAuthenticated ? t('auth.addAccountFailed') : t('auth.loginFailedCheckNsec'));
      errorHandler.log(isAuthenticated ? 'Add account failed' : 'Login failed', error, {
        context: 'LoginForm.handleSubmit',
      });
    } finally {
      setIsLoading(false);
    }
  };

  const handleBack = () => {
    navigate({ to: isAuthenticated ? '/' : '/welcome' });
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <Card className="w-full max-w-md">
        <CardHeader>
          <Button variant="ghost" size="sm" onClick={handleBack} className="w-fit -ml-2 mb-2">
            <ArrowLeft className="w-4 h-4 mr-2" />
            {t('auth.back')}
          </Button>
          <CardTitle>
            {isAuthenticated ? t('common.addAnotherAccount') : t('auth.loginTitle')}
          </CardTitle>
          <CardDescription>
            {isAuthenticated ? t('auth.addAccountDescription') : t('auth.loginDescription')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="nsec">{t('auth.nsecLabel')}</Label>
              <div className="relative">
                <Input
                  id="nsec"
                  type={showPassword ? 'text' : 'password'}
                  value={nsec}
                  onChange={(e) => setNsec(e.target.value)}
                  placeholder={t('auth.nsecPlaceholder')}
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
                  {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">{t('auth.nsecHint')}</p>
            </div>

            {!isAuthenticated && (
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
                  {t('auth.saveAccount')}
                </Label>
              </div>
            )}

            <div className="bg-warning/10 border border-warning rounded-md p-3">
              <p className="text-sm text-warning-foreground">{t('auth.nsecWarning')}</p>
            </div>

            <Button type="submit" className="w-full" disabled={isLoading}>
              {isLoading
                ? isAuthenticated
                  ? t('auth.addingAccount')
                  : t('auth.loggingIn')
                : isAuthenticated
                  ? t('common.addAnotherAccount')
                  : t('auth.login')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
