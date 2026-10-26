' INV: vbal.5.5.1.2.13, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.13 Let-coercion to and from a class, Let from Nothing to any type is error 91
' ERROR: 91
'#MODULE Wrap class
Public Property Get Value() As Long
Attribute Value.VB_UserMemId = 0
    Value = 1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim w As Wrap
    Dim n As Long
    n = w
    Debug.Print "not reached"
End Sub
