' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 GoTo a line-number label
' EXPECT: handler 5
Public Sub Main()
    On Error GoTo 100
    Error 5
    Exit Sub
100 Debug.Print "handler " & CStr(Err.Number)
End Sub
