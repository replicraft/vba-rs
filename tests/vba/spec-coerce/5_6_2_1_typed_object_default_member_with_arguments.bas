' INV: vbal.5.6.2.1, expr.default-member
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, a specific class variable with an argument list invokes its public default member with that list
' EXPECT: item=20
' EXPECT: item2=50
'#MODULE Idx class
Public Property Get Item(ByVal i As Long) As Long
Attribute Item.VB_UserMemId = 0
    Item = i * 10
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim o As New Idx
    Dim x As Long
    x = o(2)
    Debug.Print "item=" & CStr(x)
    Debug.Print "item2=" & CStr(o(5))
End Sub
