import { useCallback, useEffect, useLayoutEffect, useRef, type ReactNode, type RefObject } from "react";
import { prefersReducedMotion, SpringAnimator } from "@/viewmodels/motion";

const PAGE_OFFSET_PX = 14;
const PAGE_RESPONSE = 0.36;
const INDICATOR_RESPONSE = 0.32;
const REDUCED_FADE_MS = 200;

export function PageTransition({ pageKey, direction, children }: { pageKey: string; direction: number; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) {
      return;
    }
    if (prefersReducedMotion()) {
      element.animate?.([{ opacity: 0 }, { opacity: 1 }], { duration: REDUCED_FADE_MS, easing: "linear" });
      return;
    }
    const animator = new SpringAnimator(
      { opacity: 0, offset: PAGE_OFFSET_PX * Math.sign(direction || 1) },
      (values) => {
        element.style.opacity = String(Math.min(1, Math.max(0, values.opacity)));
        element.style.transform = Math.abs(values.offset) < 0.01 ? "" : `translate3d(0, ${values.offset}px, 0)`;
      },
      (active) => {
        element.style.willChange = active ? "transform, opacity" : "";
      },
    );
    animator.to({ opacity: 1, offset: 0 }, { response: PAGE_RESPONSE });
    return () => animator.stop();
  }, [pageKey, direction]);
  return (
    <div ref={ref} className="page-transition">
      {children}
    </div>
  );
}

export function useSlidingIndicator(
  container: RefObject<HTMLElement | null>,
  indicator: RefObject<HTMLElement | null>,
  activeKey: string,
  axis: "x" | "y",
) {
  const animator = useRef<SpringAnimator<"position" | "size"> | null>(null);

  const measure = useCallback(() => {
    const active = container.current?.querySelector<HTMLElement>("[data-active='true']");
    if (!active) {
      return null;
    }
    return axis === "x" ? { position: active.offsetLeft, size: active.offsetWidth } : { position: active.offsetTop, size: active.offsetHeight };
  }, [axis, container]);

  useLayoutEffect(() => {
    const target = measure();
    const element = indicator.current;
    if (!target || !element) {
      return;
    }
    if (!animator.current) {
      animator.current = new SpringAnimator(
        target,
        (values) => {
          element.style.transform = axis === "x" ? `translateX(${values.position}px)` : `translateY(${values.position}px)`;
          if (axis === "x") {
            element.style.width = `${values.size}px`;
          } else {
            element.style.height = `${values.size}px`;
          }
          element.style.opacity = "1";
        },
        (active) => {
          element.style.willChange = active ? "transform" : "";
        },
      );
      return;
    }
    animator.current.to(target, { response: INDICATOR_RESPONSE });
  }, [activeKey, axis, indicator, measure]);

  useEffect(() => {
    const realign = () => {
      const target = measure();
      if (target) {
        animator.current?.set(target);
      }
    };
    window.addEventListener("resize", realign);
    document.fonts?.ready.then(realign).catch(() => undefined);
    return () => {
      window.removeEventListener("resize", realign);
      animator.current?.stop();
    };
  }, [measure]);
}
