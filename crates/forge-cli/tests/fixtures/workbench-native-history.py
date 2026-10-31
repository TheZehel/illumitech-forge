#!/usr/bin/env python3
"""Historical native statuses, no model/command/turn execution capability."""
import json,sys
for line in sys.stdin:
 v=json.loads(line);method=v.get('method');p=v.get('params',{})
 if 'id' not in v:continue
 if method=='initialize':r={'userAgent':'history-fixture','platformFamily':'unix','platformOs':'macos'}
 elif method=='account/read':r={'account':{'type':'chatgpt'}}
 elif method in ['thread/start','thread/resume']:
  turns=[] if method=='thread/start' else [
   {'id':'completed-history','status':'completed','items':[{'id':'message','type':'agentMessage','text':'Historical reply'}]},
   {'id':'interrupted-history','status':'interrupted','items':[]},
   {'id':'failed-history','status':'failed','items':[],'error':{'code':'fixture_failure','password':'fixture-credential-not-for-browser'}},
   {'id':'unknown-history','status':'futureUnknown','items':[]},
   {'status':'completed','items':[]}]
  r={'thread':{'id':p.get('threadId','history-thread'),'cwd':p['cwd'],'turns':turns},'cwd':p['cwd'],'model':'native-default','sandbox':{'type':'readOnly'},'approvalPolicy':'on-request'}
 else:raise AssertionError('No execution or replay allowed: '+str(method))
 print(json.dumps({'id':v['id'],'result':r}),flush=True)
