import { useEffect, useRef, type RefObject } from "react";
import { prefersReducedMotion, SpringAnimator } from "./motion";

const REVEAL_OFFSET_PX = 18;
const REVEAL_RESPONSE = 0.42;

export function useSectionReveal<T extends HTMLElement>(): RefObject<T | null> {
  const ref = useRef<T | null>(null);

  useEffect(() => {
    const element = ref.current;
    if (!element) {
      return;
    }
    if (prefersReducedMotion() || typeof IntersectionObserver !== "function") {
      element.style.opacity = "1";
      element.style.transform = "";
      return;
    }

    const rect = element.getBoundingClientRect();
    if (rect.top < window.innerHeight * 0.92) {
      element.style.opacity = "1";
      element.style.transform = "";
      return;
    }

    let animator: SpringAnimator<"opacity" | "offset"> | null = new SpringAnimator(
      { opacity: 0.01, offset: REVEAL_OFFSET_PX },
      (values) => {
        element.style.opacity = String(Math.min(1, Math.max(0, values.opacity)));
        element.style.transform =
          Math.abs(values.offset) < 0.1 ? "" : `translate3d(0, ${values.offset.toFixed(2)}px, 0)`;
      },
      (active) => {
        element.style.willChange = active ? "transform, opacity" : "";
      },
    );

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (!entry?.isIntersecting) {
          return;
        }
        observer.disconnect();
        animator?.to(
          { opacity: 1, offset: 0 },
          { response: REVEAL_RESPONSE, damping: 1.0, epsilonDistance: 0.05, epsilonVelocity: 0.2 },
        );
      },
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" },
    );

    observer.observe(element);

    return () => {
      observer.disconnect();
      animator?.stop();
      animator = null;
      element.style.opacity = "1";
      element.style.transform = "";
      element.style.willChange = "";
    };
  }, []);

  return ref;
}
