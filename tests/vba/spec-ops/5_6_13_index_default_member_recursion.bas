' INV: vbal.5.6.13, expr.index, expr.default-member
' SOURCE: spec ms-vbal 5.6.13 a default member that takes no parameters is applied first, then the arguments go to its result
' XFAIL: wrong pending
' EXPECT: deep-5
'#MODULE Inner class
Public Property Get Item(ByVal i As Long) As String
Attribute Item.VB_UserMemId = 0
    Item = "deep-" & i
End Property
'#MODULE Outer class
Private inn As New Inner
Public Property Get Child() As Inner
Attribute Child.VB_UserMemId = 0
    Set Child = inn
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim o As New Outer
    Debug.Print o(5)
End Sub
