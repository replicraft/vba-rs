' INV: vbal.5.5.1.2.13, expr.implicit-conv, expr.default-member
' SOURCE: spec ms-vbal 5.5.1.2.13 Let-coercion to and from a class, Let from an object uses the simple data value (default member) coerced to the destination
' EXPECT: long=7
' EXPECT: str=7
' EXPECT: dbl=7
' EXPECT: variant type=Integer
' EXPECT: variant=7
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
    Dim n As Long
    Dim s As String
    Dim d As Double
    Dim x As Variant
    w.Value = 7
    n = w: Debug.Print "long=" & CStr(n)
    s = w: Debug.Print "str=" & s
    d = w: Debug.Print "dbl=" & CStr(d)
    x = w
    Debug.Print "variant type=" & TypeName(x)
    Debug.Print "variant=" & CStr(x)
End Sub
