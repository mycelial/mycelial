"use client";

export { Select, Textarea, MultiSelect} from '@mantine/core';

interface TextInputProps {
    label: string;
    name: string;
    type?: string;
    placeholder: string;
    defaultValue: string;
    onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

export function TextInput(props: TextInputProps) {
    return (
      <div>
        <label htmlFor={props.name} className="block text-sm font-medium leading-6 text-gray-900">
          {props.label}
        </label>
        <div className="mt-2">
          <input
            type={props.type || "text"}
            name={props.name}
            id={props.name}
            className="nodrag block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
            placeholder={props.placeholder}
            defaultValue={props.defaultValue}
            onChange={props.onChange}
          />
        </div>
      </div>
    )
  }