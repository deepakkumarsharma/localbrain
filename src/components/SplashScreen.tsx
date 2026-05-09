import logo from '../assets/logo.png';

interface SplashScreenProps {
  onComplete: () => void;
}

export function SplashScreen({ onComplete }: SplashScreenProps) {
  return (
    <div className="splash-root" role="status" aria-live="polite" aria-label="Loading Local Brain">
      <div className="splash-orb splash-orb-a" aria-hidden="true" />
      <div className="splash-orb splash-orb-b" aria-hidden="true" />
      <div className="splash-logo-wrap">
        <img src={logo} alt="Local Brain logo" className="splash-logo" />
      </div>
      <h1 className="splash-title">Local Brain</h1>
      <p className="splash-subtitle">Your private knowledge workspace</p>
      <div className="splash-track" aria-hidden="true">
        <div className="splash-fill" onAnimationEnd={onComplete} />
      </div>
      <p className="splash-loading">Booting local graph engine...</p>
    </div>
  );
}
