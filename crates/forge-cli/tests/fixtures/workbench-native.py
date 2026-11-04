#!/usr/bin/env python3
"""Local protocol regression fixture. Never invokes a model or command."""
import json
import sys
pending = None
thread_index = 0
root = ''
def send(v):
    print(json.dumps(v), flush=True)
def answer(v, result):
    send({'id': v['id'], 'result': result})
for line in sys.stdin:
    v = json.loads(line)
    if 'method' not in v:
        if pending and v['id'] == 'permission':
            send({'method': 'turn/completed', 'params': {'threadId': pending, 'turn': {'id':'turn', 'status':'failed' if v['result']['decision'] in ['decline','cancel'] else 'completed'}}})
            pending = None
        continue
    method = v['method']
    p = v.get('params', {})
    if 'id' not in v:
        continue
    if method == 'initialize':
        assert not p.get('capabilities',{}).get('experimentalApi')
        answer(v, {'userAgent':'fixture/1', 'platformFamily':'unix','platformOs':'macos'})
    elif method == 'account/read':
        answer(v, {'account':{'type':'chatgpt','email':'never-render@example.invalid'}})
    elif method in ['thread/start', 'thread/resume']:
        assert 'model' not in p and 'config' not in p
        assert p['sandbox'] == 'read-only' and p['approvalPolicy'] == 'on-request'
        thread_index += 1
        root = p['cwd']
        answer(v, {'thread':{'id':p.get('threadId', 'thread-'+str(thread_index)), 'cwd':root, 'turns':[]}, 'cwd':root, 'model':'native-default','sandbox':{'type':'readOnly'}, 'approvalPolicy':'on-request'})
    elif method == 'turn/start':
        text = p['input'][0]['text']
        thread = p['threadId']
        answer(v, {'turn':{'id':'turn','status':'inProgress'}})
        send({'method':'turn/started','params':{'threadId':thread,'turn':{'id':'turn'}}})
        if text == 'disconnect':
            sys.exit(0)
        if 'approval' in text:
            pending = thread
            send({'id':'permission','method':'item/commandExecution/requestApproval','params':{'threadId':thread,'turnId':'turn','reason':'Permission fixture request','command':'fixture only','availableDecisions':['accept','decline','cancel']}})
        elif text == 'wait':
            continue
        else:
            if 'outputSchema' in p:
                text = json.dumps({'schema_version':1,'id':'001-greeting','title':'Greeting','summary':'Observable greeting','tasks':[{'id':'greeting-T01','title':'Implement greeting','dependencies':[],'acceptance':[{'id':'greeting-AC1','description':'Greeting regression passes external gate','command':['cargo','test']}]}]})
            else:
                text = 'Native fixture reply'
            if 'Corrigir um bug com regressão externa.' in p['input'][0]['text'] and 'outputSchema' in p:
                verifier = "import pathlib,sys\np=pathlib.Path(sys.argv[1])/'output.txt'\nsys.exit(0 if p.exists() and p.read_text()=='verified' else 42)\n"
                text = json.dumps({'schema_version':1,'id':'058-ui-bug','title':'Correct output','summary':'Correct output with frozen external regression','tasks':[{'id':'fix','title':'Correct output','dependencies':[],'acceptance':[{'id':'preserved','description':'Preserve existing file','command':['python3','-c','from pathlib import Path; assert Path("preserved.txt").read_text()=="keep"']}]}], 'bug':{'expected':'output.txt contains verified','reproduction':'read output.txt','filename':'verify.py','content':verifier,'argv':['python3','{checks}/verify.py','{repo}']}})
            if 'INVALID_PLAN' in p['input'][0]['text'] and 'outputSchema' in p:
                text = '{"schema_version":1,"id":"bad","title":"Bad","summary":"No evidence","tasks":[]}'
            if 'CHANGED_PLAN' in p['input'][0]['text'] and 'outputSchema' in p:
                candidate = json.loads(text)
                candidate['tasks'][0]['title'] = 'Changed greeting scope'
                text = json.dumps(candidate)
            send({'method':'item/agentMessage/delta','params':{'threadId':thread,'itemId':'message','delta':text}})
            send({'method':'item/completed','params':{'threadId':thread,'item':{'id':'message','type':'agentMessage','text':text}}})
            send({'method':'turn/diff/updated','params':{'threadId':thread,'diff':'--- a/demo\n+++ b/demo\n+fixture diff (not applied)'}})
            send({'method':'thread/tokenUsage/updated','params':{'threadId':thread,'tokenUsage':{'total':{'inputTokens':12,'cachedInputTokens':2,'outputTokens':6,'totalTokens':18},'last':{'totalTokens':18}}}})
            send({'method':'turn/completed','params':{'threadId':thread,'turn':{'id':'turn','status':'completed'}}})
    elif method == 'turn/interrupt':
        answer(v,{})
        send({'method':'turn/completed','params':{'threadId':p['threadId'],'turn':{'id':p['turnId'],'status':'interrupted'}}})
    else:
        send({'id':v['id'],'error':{'code':-32601,'message':'unsupported fixture method'}})
