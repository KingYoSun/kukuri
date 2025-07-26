import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Hash, Plus, TrendingUp, Users } from "lucide-react";
import { useTopicStore, useUIStore } from "@/stores";
import { cn } from "@/lib/utils";
import { useNavigate } from "@tanstack/react-router";
import { RelayStatus } from "@/components/RelayStatus";

const categories = [
  { name: "トレンド", icon: TrendingUp },
  { name: "フォロー中", icon: Users },
];

export function Sidebar() {
  const { topics, joinedTopics, currentTopic, setCurrentTopic } = useTopicStore();
  const { sidebarOpen } = useUIStore();
  const navigate = useNavigate();

  const joinedTopicsList = joinedTopics
    .map(id => topics.get(id))
    .filter(Boolean) as NonNullable<ReturnType<typeof topics.get>>[];

  const handleTopicClick = (topicId: string) => {
    const topic = topics.get(topicId);
    if (topic) {
      setCurrentTopic(topic);
      navigate({ to: `/topics/${topicId}` });
    }
  };

  return (
    <aside 
      role="complementary" 
      className={cn(
        "border-r bg-background transition-all duration-300 overflow-hidden",
        sidebarOpen ? "w-64" : "w-0"
      )}
    >
      <div className="flex flex-col h-full w-64">
        <div className="p-4">
          <Button className="w-full" variant="default">
            <Plus className="mr-2 h-4 w-4" />
            新規投稿
          </Button>
        </div>

        <Separator />

        <div className="p-4">
          <h3 className="mb-2 text-sm font-semibold text-muted-foreground">
            カテゴリー
          </h3>
          <div className="space-y-1">
            {categories.map((category) => (
              <Button
                key={category.name}
                variant="ghost"
                className="w-full justify-start"
              >
                <category.icon className="mr-2 h-4 w-4" />
                {category.name}
              </Button>
            ))}
          </div>
        </div>

        <Separator />

        <ScrollArea className="flex-1">
          <div className="p-4">
            <h3 className="mb-2 text-sm font-semibold text-muted-foreground">
              参加中のトピック
            </h3>
            <div className="space-y-1">
              {joinedTopicsList.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  参加中のトピックはありません
                </p>
              ) : (
                joinedTopicsList.map((topic) => (
                  <Button
                    key={topic.id}
                    variant={currentTopic?.id === topic.id ? "secondary" : "ghost"}
                    className="w-full justify-start"
                    onClick={() => handleTopicClick(topic.id)}
                  >
                    <Hash className="mr-2 h-4 w-4" />
                    <span className="flex-1 text-left">{topic.name}</span>
                    <span className="text-xs text-muted-foreground">
                      {topic.memberCount}
                    </span>
                  </Button>
                ))
              )}
            </div>
          </div>
        </ScrollArea>

        <Separator />

        <div className="p-4">
          <RelayStatus />
        </div>
      </div>
    </aside>
  );
}