import React, { useState, useRef, useCallback } from 'react';
import MDEditor, { commands } from '@uiw/react-md-editor';
import { cn } from '@/lib/utils';
import { errorHandler } from '@/lib/errorHandler';
import { Upload } from 'lucide-react';
import rehypeRaw from 'rehype-raw';
import remarkGfm from 'remark-gfm';
import MarkdownPreview from './MarkdownPreview';

interface MarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  height?: number;
  preview?: 'live' | 'edit' | 'preview';
  hideToolbar?: boolean;
  onImageUpload?: (file: File) => Promise<string>;
  maxLength?: number;
}

const MarkdownEditor: React.FC<MarkdownEditorProps> = ({
  value,
  onChange,
  placeholder = 'Write your post content here...',
  className,
  height = 300,
  preview = 'live',
  hideToolbar = false,
  onImageUpload,
  maxLength,
}) => {
  const [uploadingImage, setUploadingImage] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleImageClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file || !onImageUpload) return;

      // Validate file type
      const validTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/gif', 'image/webp'];
      if (!validTypes.includes(file.type)) {
        errorHandler.log('Invalid file type. Please upload an image.', undefined, { showToast: true });
        return;
      }

      // Validate file size (max 10MB)
      const maxSize = 10 * 1024 * 1024;
      if (file.size > maxSize) {
        errorHandler.log('File size too large. Maximum size is 10MB.', undefined, { showToast: true });
        return;
      }

      setUploadingImage(true);
      try {
        const imageUrl = await onImageUpload(file);
        const imageMarkdown = `![${file.name}](${imageUrl})`;
        
        // Insert image at cursor position or at the end
        const textarea = document.querySelector('.w-md-editor-text-input') as HTMLTextAreaElement;
        if (textarea) {
          const start = textarea.selectionStart;
          const end = textarea.selectionEnd;
          const newValue = value.substring(0, start) + imageMarkdown + value.substring(end);
          onChange(newValue);
          
          // Reset cursor position after image
          setTimeout(() => {
            textarea.setSelectionRange(start + imageMarkdown.length, start + imageMarkdown.length);
            textarea.focus();
          }, 0);
        } else {
          onChange(value + '\n' + imageMarkdown);
        }
      } catch (error) {
        errorHandler.log('Failed to upload image', error, { 
          context: 'Image upload failed',
          showToast: true 
        });
      } finally {
        setUploadingImage(false);
        // Reset file input
        if (fileInputRef.current) {
          fileInputRef.current.value = '';
        }
      }
    },
    [value, onChange, onImageUpload]
  );

  const handleChange = useCallback(
    (newValue?: string) => {
      const val = newValue || '';
      if (maxLength && val.length > maxLength) {
        return;
      }
      onChange(val);
    },
    [onChange, maxLength]
  );

  const customCommands = React.useMemo(() => {
    const defaultCommands = [
      commands.bold,
      commands.italic,
      commands.strikethrough,
      commands.hr,
      commands.title,
      commands.divider,
      commands.link,
      commands.quote,
      commands.code,
      commands.image,
      commands.divider,
      commands.unorderedListCommand,
      commands.orderedListCommand,
      commands.checkedListCommand,
    ].filter(Boolean);
    
    if (onImageUpload) {
      // Add custom image upload command
      const imageCommand = {
        name: 'image-upload',
        keyCommand: 'image-upload',
        buttonProps: { 'aria-label': 'Upload image', title: 'Upload image' },
        icon: (
          <span style={{ display: 'flex', alignItems: 'center' }}>
            {uploadingImage ? (
              <span className="animate-spin">↻</span>
            ) : (
              <Upload size={12} />
            )}
          </span>
        ),
        execute: () => {
          handleImageClick();
        },
      };
      
      // Insert after the image command
      const imageIndex = defaultCommands.findIndex((cmd) => cmd && 'name' in cmd && cmd.name === 'image');
      if (imageIndex >= 0) {
        const newCommands = [...defaultCommands];
        newCommands.splice(imageIndex + 1, 0, imageCommand);
        return newCommands;
      }
    }
    
    return defaultCommands;
  }, [onImageUpload, uploadingImage, handleImageClick]);

  return (
    <div className={cn('relative', className)}>
      <MDEditor
        value={value}
        onChange={handleChange}
        preview={preview}
        height={height}
        hideToolbar={hideToolbar}
        commands={customCommands}
        textareaProps={{
          placeholder,
        }}
        previewOptions={{
          remarkPlugins: [[remarkGfm]],
          rehypePlugins: [[rehypeRaw]],
        }}
        components={{
          preview: (source: { value?: string } | string) => {
            const text = typeof source === 'string' ? source : (source?.value || '');
            return <MarkdownPreview content={text} />;
          },
        }}
      />
      
      {/* Hidden file input for image uploads */}
      {onImageUpload && (
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          onChange={handleFileChange}
          className="hidden"
          aria-label="Upload image file"
        />
      )}
      
      {/* Character count */}
      {maxLength && (
        <div className="absolute bottom-2 right-2 text-xs text-muted-foreground">
          {value.length} / {maxLength}
        </div>
      )}
    </div>
  );
};

export default MarkdownEditor;