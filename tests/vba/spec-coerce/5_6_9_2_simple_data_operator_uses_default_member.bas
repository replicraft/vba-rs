' INV: vbal.5.6.9.2, expr.default-member, op.add
' SOURCE: spec ms-vbal 5.6.9.2 Simple data operators, operands are evaluated to simple data values before the operator runs
' EXPECT: add=8
' EXPECT: variant add=8
' EXPECT: both=14
' EXPECT: type=Integer
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
    Debug.Print "add=" & CStr(w + 1)
    Set x = w
    Debug.Print "variant add=" & CStr(x + 1)
    Debug.Print "both=" & CStr(w + w)
    Debug.Print "type=" & TypeName(w + 1)
End Sub
