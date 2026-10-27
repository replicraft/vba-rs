' INV: vbal.5.2.4.3
' SOURCE: spec ms-vbal 5.2.4.3 an event name may equal a module variable, constant or procedure name
' XFAIL: wrong not implemented: events share the symbol table with variables, constants and procedures, so an event named like a member is a duplicate declaration (the specification text allows it; whether VBA accepts it is unconfirmed)
' EXPECT: ok
Public Sub Main()
    Dim s As New Src
    s.Go
    Debug.Print "ok"
End Sub
'#MODULE Src class
Public Event Value()
Public Event Limit()
Public Event Go()
Private Value As Long
Private Const Limit As Long = 1
Public Sub Go()
End Sub
