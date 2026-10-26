' INV: vbal.5.6.13, expr.index, expr.default-member
' SOURCE: spec ms-vbal 5.6.13 index on a specific class applies the argument list to its default member
' EXPECT: item-3
' EXPECT: item-4
' EXPECT: 77
'#MODULE Bag class
Private store As Long
Public Property Get Item(ByVal i As Long) As String
Attribute Item.VB_UserMemId = 0
    Item = "item-" & i
End Property
Public Property Get Other(ByVal i As Long) As Long
    Other = store + i
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Bag, v As Variant
    Debug.Print b(3)
    Set v = b
    Debug.Print v(4)
    Debug.Print CStr(b.Other(77))
End Sub
