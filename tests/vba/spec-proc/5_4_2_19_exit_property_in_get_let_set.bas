' INV: vbal.5.4.2.19, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.19 Exit Property ends a property procedure without executing later statements
' EXPECT: get=5
' EXPECT: let-stopped x=1
'#MODULE Cls class
Public X As Long
Public Property Get P() As Long
    P = 5
    Exit Property
    P = 99
End Property
Public Property Let P(ByVal v As Long)
    X = 1
    Exit Property
    X = v
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Cls
    Debug.Print "get=" & CStr(c.P)
    c.P = 42
    Debug.Print "let-stopped x=" & CStr(c.X)
End Sub
