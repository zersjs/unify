import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { prefersReducedMotion } from "./motion";

export interface NavigationViewModel {
  readonly scrolled: boolean;
  readonly mobileOpen: boolean;
  readonly menuButtonRef: RefObject<HTMLButtonElement | null>;
  readonly toggleMobileMenu: () => void;
  readonly closeMobileMenu: () => void;
  readonly navigateToSection: (sectionId: string) => void;
}

export function useNavigationViewModel(): NavigationViewModel {
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const menuButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    const syncScroll = () => {
      setScrolled(window.scrollY > 12);
    };
    syncScroll();
    window.addEventListener("scroll", syncScroll, { passive: true });
    return () => {
      window.removeEventListener("scroll", syncScroll);
    };
  }, []);

  useEffect(() => {
    if (!mobileOpen) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMobileOpen(false);
        menuButtonRef.current?.focus();
      }
    };
    const onResize = () => {
      if (window.innerWidth >= 768) {
        setMobileOpen(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("resize", onResize);
    };
  }, [mobileOpen]);

  const toggleMobileMenu = useCallback(() => {
    setMobileOpen((prev) => !prev);
  }, []);

  const closeMobileMenu = useCallback(() => {
    setMobileOpen(false);
  }, []);

  const navigateToSection = useCallback((sectionId: string) => {
    setMobileOpen(false);
    const target = document.getElementById(sectionId);
    if (!target) {
      return;
    }
    target.scrollIntoView({
      behavior: prefersReducedMotion() ? "auto" : "smooth",
      block: "start",
    });
    if (typeof window.history?.replaceState === "function") {
      window.history.replaceState(null, "", `#${sectionId}`);
    }
  }, []);

  return {
    scrolled,
    mobileOpen,
    menuButtonRef,
    toggleMobileMenu,
    closeMobileMenu,
    navigateToSection,
  };
}
