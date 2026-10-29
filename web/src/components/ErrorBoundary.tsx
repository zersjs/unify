import { Component, type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryProps {
  readonly fallbackTitle: string;
  readonly retryLabel: string;
  readonly children: ReactNode;
}

interface ErrorBoundaryState {
  readonly hasError: boolean;
}

export class LandingErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  public override state: ErrorBoundaryState = { hasError: false };

  public static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  public override componentDidCatch(_error: Error, _info: ErrorInfo): void {
    void _error;
    void _info;
  }

  private readonly handleReset = (): void => {
    this.setState({ hasError: false });
  };

  public override render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div className="error-fallback" role="alert">
          <p className="error-fallback-title">{this.props.fallbackTitle}</p>
          <button type="button" className="btn-primary" onClick={this.handleReset}>
            {this.props.retryLabel}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
