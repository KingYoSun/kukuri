import React, { useState } from 'react';
import { format, isFuture, isToday, isTomorrow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { DayPicker } from 'react-day-picker';
import 'react-day-picker/dist/style.css';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { cn } from '@/lib/utils';
import { Calendar, Clock, X } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

interface PostSchedulerProps {
  scheduledDate: Date | null;
  onSchedule: (date: Date | null) => void;
  className?: string;
}

const PostScheduler: React.FC<PostSchedulerProps> = ({
  scheduledDate,
  onSchedule,
  className,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [selectedDate, setSelectedDate] = useState<Date | undefined>(scheduledDate || undefined);
  const [selectedHour, setSelectedHour] = useState(
    scheduledDate ? format(scheduledDate, 'HH') : '09'
  );
  const [selectedMinute, setSelectedMinute] = useState(
    scheduledDate ? format(scheduledDate, 'mm') : '00'
  );

  const handleDateSelect = (date: Date | undefined) => {
    if (!date) {
      onSchedule(null);
      setIsOpen(false);
      return;
    }

    // Combine date with time
    const scheduled = new Date(date);
    scheduled.setHours(parseInt(selectedHour, 10));
    scheduled.setMinutes(parseInt(selectedMinute, 10));
    scheduled.setSeconds(0);
    scheduled.setMilliseconds(0);

    setSelectedDate(date);
    onSchedule(scheduled);
  };

  const handleTimeChange = (hour: string, minute: string) => {
    setSelectedHour(hour);
    setSelectedMinute(minute);

    if (selectedDate) {
      const scheduled = new Date(selectedDate);
      scheduled.setHours(parseInt(hour, 10));
      scheduled.setMinutes(parseInt(minute, 10));
      scheduled.setSeconds(0);
      scheduled.setMilliseconds(0);
      onSchedule(scheduled);
    }
  };

  const handleQuickSelect = (option: 'today' | 'tomorrow') => {
    const date = new Date();
    if (option === 'tomorrow') {
      date.setDate(date.getDate() + 1);
    }
    
    // Set default time for quick selections
    date.setHours(parseInt(selectedHour, 10));
    date.setMinutes(parseInt(selectedMinute, 10));
    date.setSeconds(0);
    date.setMilliseconds(0);

    setSelectedDate(date);
    onSchedule(date);
  };

  const clearSchedule = () => {
    setSelectedDate(undefined);
    onSchedule(null);
    setIsOpen(false);
  };

  const getScheduleText = () => {
    if (!scheduledDate) return '予約投稿';

    const dateStr = format(scheduledDate, 'M月d日 (E)', { locale: ja });
    const timeStr = format(scheduledDate, 'HH:mm');

    if (isToday(scheduledDate)) {
      return `今日 ${timeStr}`;
    } else if (isTomorrow(scheduledDate)) {
      return `明日 ${timeStr}`;
    }

    return `${dateStr} ${timeStr}`;
  };

  // Generate hour options (00-23)
  const hours = Array.from({ length: 24 }, (_, i) => i.toString().padStart(2, '0'));
  
  // Generate minute options (00, 15, 30, 45)
  const minutes = ['00', '15', '30', '45'];

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>
        <Button
          variant={scheduledDate ? 'default' : 'outline'}
          size="sm"
          className={cn('gap-2', className)}
        >
          {scheduledDate ? <Clock className="w-4 h-4" /> : <Calendar className="w-4 h-4" />}
          {getScheduleText()}
          {scheduledDate && (
            <X
              className="w-4 h-4 ml-1"
              onClick={(e) => {
                e.stopPropagation();
                clearSchedule();
              }}
            />
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto p-0" align="start">
        <div className="p-3 space-y-3">
          {/* Quick select buttons */}
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => handleQuickSelect('today')}
              className="flex-1"
            >
              今日
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => handleQuickSelect('tomorrow')}
              className="flex-1"
            >
              明日
            </Button>
          </div>

          {/* Calendar */}
          <DayPicker
            mode="single"
            selected={selectedDate}
            onSelect={handleDateSelect}
            disabled={(date) => !isFuture(date) && !isToday(date)}
            locale={ja}
            className="border rounded-md"
            classNames={{
              months: 'flex flex-col sm:flex-row space-y-4 sm:space-x-4 sm:space-y-0',
              month: 'space-y-4',
              caption: 'flex justify-center pt-1 relative items-center',
              caption_label: 'text-sm font-medium',
              nav: 'space-x-1 flex items-center',
              nav_button: cn(
                'h-7 w-7 bg-transparent p-0 opacity-50 hover:opacity-100',
                'hover:bg-accent hover:text-accent-foreground rounded-md'
              ),
              nav_button_previous: 'absolute left-1',
              nav_button_next: 'absolute right-1',
              table: 'w-full border-collapse space-y-1',
              head_row: 'flex',
              head_cell: 'text-muted-foreground rounded-md w-9 font-normal text-[0.8rem]',
              row: 'flex w-full mt-2',
              cell: 'text-center text-sm p-0 relative [&:has([aria-selected])]:bg-accent first:[&:has([aria-selected])]:rounded-l-md last:[&:has([aria-selected])]:rounded-r-md focus-within:relative focus-within:z-20',
              day: cn(
                'h-9 w-9 p-0 font-normal',
                'hover:bg-accent hover:text-accent-foreground rounded-md',
                'focus:bg-accent focus:text-accent-foreground'
              ),
              day_selected: 'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground focus:bg-primary focus:text-primary-foreground',
              day_today: 'bg-accent text-accent-foreground',
              day_outside: 'text-muted-foreground opacity-50',
              day_disabled: 'text-muted-foreground opacity-50 cursor-not-allowed',
              day_hidden: 'invisible',
            }}
          />

          {/* Time selector */}
          <div className="space-y-2">
            <Label className="text-sm">時刻</Label>
            <div className="flex gap-2 items-center">
              <Select
                value={selectedHour}
                onValueChange={(value) => handleTimeChange(value, selectedMinute)}
              >
                <SelectTrigger className="w-20">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {hours.map((hour) => (
                    <SelectItem key={hour} value={hour}>
                      {hour}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <span className="text-sm">:</span>
              <Select
                value={selectedMinute}
                onValueChange={(value) => handleTimeChange(selectedHour, value)}
              >
                <SelectTrigger className="w-20">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {minutes.map((minute) => (
                    <SelectItem key={minute} value={minute}>
                      {minute}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Action buttons */}
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={clearSchedule}
              className="flex-1"
            >
              クリア
            </Button>
            <Button
              size="sm"
              onClick={() => setIsOpen(false)}
              className="flex-1"
              disabled={!selectedDate}
            >
              設定
            </Button>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
};

export default PostScheduler;