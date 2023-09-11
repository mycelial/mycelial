"use client";

import React, { Fragment, useEffect, useState } from "react";

import { Listbox, Transition } from "@headlessui/react";
import { CheckIcon, ChevronUpDownIcon } from "@heroicons/react/20/solid";
export { MultiSelect } from "@mantine/core";

const getRandomString = () => {
  return String(Date.now().toString(32) + Math.random().toString(16)).replace(
    /\./g,
    "",
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
        className="block text-sm font-medium text-gray-900"
      >
        {props.label}
      </label>
      <div className="">
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
      <div className="">
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

interface SelectProps {
  label: string;
  name: string;
  type?: string;
  placeholder?: string;
  defaultValue?: string;
  onChange: (value: string) => void;
  options: string[];
}

function classNames(...classes: (string | boolean)[]) {
  return classes.filter(Boolean).join(" ");
}

export const Select: React.FC<SelectProps> = (props) => {
  const id = React.useRef<string>("");
  useEffect(() => {
    id.current = getId(props.name);
  }, [props.name]);

  const [selected, setSelected] = useState(props.defaultValue);
  useEffect(() => {
    props.onChange(selected || "");
  }, [selected]);

  return (
    <div>
      <Listbox value={selected} onChange={setSelected}>
        {({ open }) => (
          <>
            <Listbox.Label className="nodrag block text-sm font-medium leading-6 text-gray-900">
              {props.label}
            </Listbox.Label>
            <div className="relative">
              <Listbox.Button className="relative w-full cursor-default rounded-md bg-white py-1.5 pl-3 pr-10 text-left text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:outline-none focus:ring-2 focus:ring-indigo-600 sm:text-sm sm:leading-6">
                <span className="block truncate">{selected}</span>
                <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
                  <ChevronUpDownIcon
                    className="h-5 w-5 text-gray-400"
                    aria-hidden="true"
                  />
                </span>
              </Listbox.Button>

              <Transition
                show={open}
                as={Fragment}
                leave="transition ease-in duration-100"
                leaveFrom="opacity-100"
                leaveTo="opacity-0"
              >
                <Listbox.Options className="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm">
                  {props.options.map((option) => (
                    <Listbox.Option
                      key={option}
                      className={({ active }) =>
                        classNames(
                          active ? "bg-indigo-600 text-white" : "text-gray-900",
                          "relative cursor-default select-none py-2 pl-3 pr-9",
                        )
                      }
                      value={option}
                    >
                      {({ selected, active }) => (
                        <>
                          <span
                            className={classNames(
                              selected ? "font-semibold" : "font-normal",
                              "block truncate",
                            )}
                          >
                            {option}
                          </span>

                          {selected ? (
                            <span
                              className={classNames(
                                active ? "text-white" : "text-indigo-600",
                                "absolute inset-y-0 right-0 flex items-center pr-4",
                              )}
                            >
                              <CheckIcon
                                className="h-5 w-5"
                                aria-hidden="true"
                              />
                            </span>
                          ) : null}
                        </>
                      )}
                    </Listbox.Option>
                  ))}
                </Listbox.Options>
              </Transition>
            </div>
          </>
        )}
      </Listbox>
    </div>
  );
};
