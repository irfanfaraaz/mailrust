"use client";
import { ActionForm } from "@/components/action-form";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";

type Props = {
  trigger: string;
  title: string;
  description: string;
  confirm: string;
  action: () => unknown;
  /** Label for the button that closes the dialog without acting. */
  dismiss?: string;
  triggerClassName?: string;
  /** Toast shown when the action succeeds; an `{ error }` result is toasted as an error. */
  done?: string;
  /** "default" for actions that are safe to undo, like Resume. */
  variant?: "destructive" | "default";
};

/** A trigger that opens a confirm dialog; the dialog's form runs the server action. */
export function ConfirmAction({ trigger, title, description, confirm, action, dismiss = "Cancel", triggerClassName, done, variant = "destructive" }: Props) {
  return (
    <Dialog>
      <DialogTrigger className={triggerClassName ?? buttonVariants({ variant: "outline", size: "sm" })}>{trigger}</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose className={buttonVariants({ variant: "outline" })}>{dismiss}</DialogClose>
          <ActionForm action={action} done={done}>
            <button type="submit" className={buttonVariants({ variant })}>{confirm}</button>
          </ActionForm>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
