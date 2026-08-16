import { cn } from '../lib/utils'
import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  PropsWithChildren,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from 'react'

export function Button({
  className,
  variant = 'default',
  type = 'button',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'default' | 'ghost' | 'danger' }) {
  return (
    <button
      type={type}
      className={cn(
        'inline-flex items-center justify-center rounded-md px-3 py-2 text-sm font-medium transition disabled:opacity-50',
        variant === 'default' && 'bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)]',
        variant === 'ghost' && 'bg-transparent text-[var(--text)] hover:bg-[var(--bg-muted)]',
        variant === 'danger' && 'bg-[var(--danger)] text-white hover:opacity-90',
        className,
      )}
      {...props}
    />
  )
}

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={cn(
        'w-full rounded-md border border-[var(--border)] bg-[var(--bg)] px-3 py-2 text-sm text-[var(--text)] outline-none focus:ring-2 focus:ring-[var(--ring)]',
        className,
      )}
      {...props}
    />
  )
}

export function Textarea({ className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        'w-full rounded-md border border-[var(--border)] bg-[var(--bg)] px-3 py-2 font-mono text-xs leading-relaxed text-[var(--text)] outline-none focus:ring-2 focus:ring-[var(--ring)]',
        className,
      )}
      {...props}
    />
  )
}

export function Select({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={cn(
        'w-full rounded-md border border-[var(--border)] bg-[var(--bg)] px-3 py-2 text-sm text-[var(--text)] outline-none focus:ring-2 focus:ring-[var(--ring)]',
        className,
      )}
      {...props}
    />
  )
}

export function Card({ children, className }: PropsWithChildren<{ className?: string }>) {
  return (
    <div
      className={cn(
        'rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)]/90 p-5 shadow-sm backdrop-blur',
        className,
      )}
    >
      {children}
    </div>
  )
}

export function Label({ children }: PropsWithChildren) {
  return <label className="mb-1 block text-sm text-[var(--text-muted)]">{children}</label>
}

export function Badge({ children }: PropsWithChildren) {
  return (
    <span className="rounded-full bg-[var(--bg-muted)] px-2 py-0.5 text-xs text-[var(--text-muted)]">
      {children}
    </span>
  )
}
