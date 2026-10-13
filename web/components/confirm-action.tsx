import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";

type Props = {
  trigger: string;
  title: string;
  description: string;
  confirm: string;
  action: () => void | Promise<void>;
  /** Label for the button that closes the dialog without acting. */
  dismiss?: string;
  triggerClassName?: string;
};

/** A trigger that opens a confirm dialog; the dialog's form runs the server action. */
export function ConfirmAction({ trigger, title, description, confirm, action, dismiss = "Cancel", triggerClassName }: Props) {
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
          <form action={action}>
            <button type="submit" className={buttonVariants({ variant: "destructive" })}>{confirm}</button>
          </form>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
