' INV: vbal.5.6.14
' SOURCE: spec ms-vbal 5.6.14 dictionary access is an index expression with the name as a String argument
' EXPECT: got:alpha
' EXPECT: got:beta
' EXPECT: got:gamma
'#MODULE Lookup class
Public Property Get Item(ByVal key As String) As String
Attribute Item.VB_UserMemId = 0
    Item = "got:" & key
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim d As New Lookup, v As Variant, o As Object
    Debug.Print d!alpha
    Set o = d
    Debug.Print o!beta
    Set v = d
    Debug.Print v!gamma
End Sub
