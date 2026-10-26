' INV: vbal.5.6.2.1, expr.default-member
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, a Variant holding an object followed by an argument list raises error 9
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: a Variant holding an object followed by an argument list calls the object's default member (the Learn page on default members documents it) instead of raising error 9
' ERROR: 9
'#MODULE Idx class
Public Property Get Item(ByVal i As Long) As Long
Attribute Item.VB_UserMemId = 0
    Item = i * 10
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim v As Variant
    Dim x As Long
    Set v = New Idx
    x = v(2)
    Debug.Print "not reached"
End Sub
