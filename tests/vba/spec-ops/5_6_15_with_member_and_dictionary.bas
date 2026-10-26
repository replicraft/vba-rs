' INV: vbal.5.6.15, stmt.with
' SOURCE: spec ms-vbal 5.6.15 With expressions use the innermost enclosing With block variable
' EXPECT: 4
' EXPECT: got:k1
' EXPECT: 9
' EXPECT: 4
'#MODULE Lookup class
Public N As Long
Public Property Get Item(ByVal key As String) As String
Attribute Item.VB_UserMemId = 0
    Item = "got:" & key
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Lookup, b As New Lookup
    a.N = 4
    b.N = 9
    With a
        Debug.Print CStr(.N)
        Debug.Print !k1
        With b
            Debug.Print CStr(.N)
        End With
        Debug.Print CStr(.N)
    End With
End Sub
