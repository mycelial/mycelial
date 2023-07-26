"use client";

import React, { useEffect } from "react";

export { Select, Textarea, MultiSelect } from "@mantine/core";

const getRandomString = () => {
  return String(Date.now().toString(32) + Math.random().toString(16)).replace(
    /\./g,
    ""
  );
};

const getId = (name: string) => `${name}_${getRandomString()}`;

interface TextInputProps {
  label: string;
  name: string;
  type?: string;
  placeholder: string;
  defaultValue: string;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

export const TextInput: React.FC<TextInputProps> = (props) => {
  const id = React.useRef<string>("");
  useEffect(() => {
    id.current = getId(props.name);
  }, [props.name]);

  return (
    <div>
      <label
        style={{ cursor: "inherit" }}
        htmlFor={id.current}
        className="block text-sm font-medium leading-6 text-gray-900"
      >
        {props.label}
      </label>
      <div className="mt-2">
        <input
          type={props.type || "text"}
          name={props.name}
          id={id.current}
          className="nodrag block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
          placeholder={props.placeholder}
          defaultValue={props.defaultValue}
          onChange={props.onChange}
        />
      </div>
    </div>
  );
};

interface TextAreaProps {
  label: string;
  name: string;
  type?: string;
  placeholder?: string;
  defaultValue?: string;
  onChange: (event: React.ChangeEvent<HTMLTextAreaElement>) => void;
}

export const TextArea: React.FC<TextAreaProps> = (props) => {
  const id = React.useRef<string>("");
  useEffect(() => {
    id.current = getId(props.name);
  }, [props.name]);

  return (
    <div>
      <label
        style={{ cursor: "inherit" }}
        htmlFor={id.current}
        className="block text-sm font-medium leading-6 text-gray-900"
      >
        {props.label}
      </label>
      <div className="mt-2">
        <textarea
          rows={4}
          name={props.name}
          id={id.current}
          className="nodrag block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
          defaultValue={props.defaultValue || ""}
          placeholder={props.placeholder || ""}
          onChange={props.onChange}
        />
      </div>
    </div>
  );
};
