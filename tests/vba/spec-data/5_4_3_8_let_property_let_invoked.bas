' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 Property Let receives the coerced value
' EXPECT: let(5)
' EXPECT: 5
'#MODULE C1 class
Private m As Long
Public Property Let P(ByVal v As Long)
    Debug.Print "let(" & CStr(v) & ")"
    m = v
End Property
Public Property Get P() As Long
    P = m
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New C1
    c.P = "5"
    Debug.Print CStr(c.P)
End Sub
