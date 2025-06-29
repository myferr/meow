"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { ArrowRight } from "lucide-react";

export default function Waitlist() {
  const [subscribed, setSubscribed] = useState(false);

  const featuresMap = [
    { status: "complete", label: "Scrollback (PgUp/PgDn support)" },
    {
      status: "complete",
      label: "Input history (arrow key navigation)",
    },
    {
      status: "not-in-progress",
      label: "Word wrap & terminal overflow handling",
    },
    { status: "not-in-progress", label: "Auto-reconnect on disconnect" },
    {
      status: "not-in-progress",
      label: "Graceful error handling (no panics or unwraps)",
    },
    {
      status: "not-in-progress",
      label: "Config file support (`~/.meow/config.toml`)",
    },
    { status: "not-in-progress", label: "Windows and cross-platform builds" },
    {
      status: "not-in-progress",
      label: "Clean shutdown (e.g. Ctrl+C support)",
    },
  ];

  const statusColorMap: Record<string, string> = {
    complete: "bg-green-600",
    inprogress: "bg-blue-600 animate-pulse",
    "not-in-progress": "bg-gray-700 opacity-50 rounded-full",
  };

  useEffect(() => {
    const subscribed_localStorage = localStorage.getItem("subscribed");
    if (subscribed && subscribed_localStorage != "yes") {
      localStorage.setItem("subscribed", "yes");
    }
    if (subscribed_localStorage == "yes") {
      setSubscribed(true);
    }
  });

  return (
    <main className="min-h-screen text-left flex items-center justify-center p-8 font-mono">
      <div className="flex w-full max-w-5xl">
        {/* Left Section */}
        <div className="w-full max-w-md space-y-4">
          <h1 className="text-2xl font-bold">meow is coming soon</h1>
          <p className="text-sm text-gray-500">
            A modern terminal IRC client. Join the waitlist for early access.
          </p>
          {subscribed ? (
            <div className="p-4 border rounded bg-green-50 text-green-700">
              <p>Thanks for subscribing! We'll keep you posted</p>
            </div>
          ) : (
            <div>
              <form
                action="https://buttondown.email/api/emails/embed-subscribe/meowirc"
                method="post"
                target="popupwindow"
                onSubmit={(e) => {
                  window.open(
                    "https://buttondown.email/meowirc",
                    "popupwindow",
                  );
                  setSubscribed(true);
                }}
                className="flex flex-col gap-3"
              >
                <button
                  type="submit"
                  className="bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 hover:cursor-pointer"
                >
                  <p className="ml-3 flex items-center gap-2">
                    <ArrowRight size={12} /> Join Waitlist
                  </p>
                </button>
                <p className="text-xs text-gray-400">
                  You’ll only get occasional updates — no spam. Powered by{" "}
                  <Link
                    href={"https://buttondown.email"}
                    className="text-blue-300"
                  >
                    <b>buttondown.email</b>
                  </Link>
                </p>
              </form>
            </div>
          )}
          <div className="border-stone-300 border-t py-3">
            <Link
              href={"https://github.com/myferr/meow"}
              target="_blank"
              className="hover:before:content-['->'] hover:before:mr-1 text-blue-600 hover:underline hover:text-blue-700"
            >
              github
            </Link>
          </div>
        </div>

        {/* Right Section */}
        <div className="w-full max-w-md ml-82 space-y-4">
          <h2 className="text-xl font-bold">todo.md</h2>
          <ul className="flex flex-col gap-2">
            {featuresMap.map((i) => (
              <li
                key={i.label}
                className="flex gap-1.5 items-center border-stone-200 pb-2 border-b text-sm"
              >
                <div
                  className={`w-2 h-2 ${statusColorMap[i.status] || "bg-gray-400"}`}
                />{" "}
                {i.label}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </main>
  );
}
