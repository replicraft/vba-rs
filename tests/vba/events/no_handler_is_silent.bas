' INV: stmt.event, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 RaiseEvent statement invokes only declared handlers
' EXPECT: ok
'#MODULE Source class
Public Event Fired(ByVal n As Long)
Public Sub Fire(ByVal n As Long)
    RaiseEvent Fired(n)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim s As New Source
    s.Fire 1
    Debug.Print "ok"
End Sub
