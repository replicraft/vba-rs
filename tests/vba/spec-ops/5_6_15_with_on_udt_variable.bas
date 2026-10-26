' INV: vbal.5.6.15, stmt.with
' SOURCE: spec ms-vbal 5.6.15 With member access resolves as if the With variable were the l-expression (UDT field)
' EXPECT: 3
' EXPECT: 3
Private Type P
    X As Long
End Type
Public Sub Main()
    Dim u As P
    With u
        .X = 3
        Debug.Print CStr(.X)
    End With
    Debug.Print CStr(u.X)
End Sub
