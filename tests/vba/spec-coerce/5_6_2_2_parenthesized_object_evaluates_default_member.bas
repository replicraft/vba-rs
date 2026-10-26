' INV: vbal.5.6.2.2, vbal.5.6.6, expr.default-member
' SOURCE: spec ms-vbal 5.6.6 Parenthesized expressions, the parenthesized object evaluates to the simple data value of its default member
' EXPECT: type=Integer
' EXPECT: value=7
' EXPECT: sum=8
'#MODULE Wrap class
Private v As Integer
Public Property Get Value() As Integer
Attribute Value.VB_UserMemId = 0
    Value = v
End Property
Public Property Let Value(ByVal n As Integer)
    v = n
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim w As New Wrap
    Dim x As Variant
    w.Value = 7
    x = (w)
    Debug.Print "type=" & TypeName(x)
    Debug.Print "value=" & CStr(x)
    Debug.Print "sum=" & CStr((w) + 1)
End Sub
