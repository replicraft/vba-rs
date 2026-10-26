' INV: vbal.5.4.4, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Goto policy sets the policy to Default; an error in the handler propagates
' EXPECT: inner handler
' EXPECT: outer handler 13
Private Sub Inner()
    On Error GoTo h
    Error 5
    Exit Sub
h:
    Debug.Print "inner handler"
    Error 13
End Sub
Public Sub Main()
    On Error GoTo oh
    Inner
    Exit Sub
oh:
    Debug.Print "outer handler " & CStr(Err.Number)
End Sub
