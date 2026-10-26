' INV: stmt.event, stmt.raiseevent
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/raiseevent-statement
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
