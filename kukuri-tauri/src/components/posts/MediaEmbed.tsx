import React, { useMemo } from 'react';
import { cn } from '@/lib/utils';

interface MediaEmbedProps {
  url: string;
  className?: string;
}

interface EmbedConfig {
  pattern: RegExp;
  getEmbedUrl: (match: RegExpMatchArray) => string;
  aspectRatio?: string;
}

const embedConfigs: Record<string, EmbedConfig> = {
  youtube: {
    pattern: /(?:youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/embed\/)([a-zA-Z0-9_-]+)/,
    getEmbedUrl: (match) => `https://www.youtube.com/embed/${match[1]}`,
    aspectRatio: '16/9',
  },
  vimeo: {
    pattern: /vimeo\.com\/(\d+)/,
    getEmbedUrl: (match) => `https://player.vimeo.com/video/${match[1]}`,
    aspectRatio: '16/9',
  },
  twitter: {
    pattern: /twitter\.com\/\w+\/status\/(\d+)|x\.com\/\w+\/status\/(\d+)/,
    getEmbedUrl: (match) => {
      const tweetId = match[1] || match[2];
      return `https://platform.twitter.com/embed/Tweet.html?id=${tweetId}`;
    },
    aspectRatio: '1/1',
  },
};

const MediaEmbed: React.FC<MediaEmbedProps> = ({ url, className }) => {
  const embedData = useMemo(() => {
    for (const [platform, config] of Object.entries(embedConfigs)) {
      const match = url.match(config.pattern);
      if (match) {
        return {
          platform,
          embedUrl: config.getEmbedUrl(match),
          aspectRatio: config.aspectRatio || '16/9',
        };
      }
    }
    return null;
  }, [url]);

  if (!embedData) {
    // If not a recognized embed, check if it's an image
    const imagePattern = /\.(jpg|jpeg|png|gif|webp|svg)$/i;
    if (imagePattern.test(url)) {
      return (
        <img
          src={url}
          alt="Embedded image"
          className={cn('max-w-full h-auto rounded-lg', className)}
          loading="lazy"
        />
      );
    }

    // Return a link for unrecognized URLs
    return (
      <a
        href={url}
        target="_blank"
        rel="noopener noreferrer"
        className={cn('text-primary hover:underline', className)}
      >
        {url}
      </a>
    );
  }

  return (
    <div
      className={cn('relative w-full overflow-hidden rounded-lg', className)}
      style={{ aspectRatio: embedData.aspectRatio }}
    >
      <iframe
        src={embedData.embedUrl}
        className="absolute inset-0 w-full h-full"
        frameBorder="0"
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
        allowFullScreen
        loading="lazy"
        title={`Embedded ${embedData.platform} content`}
      />
    </div>
  );
};

export default MediaEmbed;
