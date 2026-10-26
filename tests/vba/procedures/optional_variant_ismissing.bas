' INV: decl.optional, func.ismissing
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ismissing-function
' EXPECT: missing
' EXPECT: got 5
Function F(Optional v As Variant) As String
    If IsMissing(v) Then F = "missing" Else F = "got " & v
End Function
Public Sub Main()
    Debug.Print F()
    Debug.Print F(5)
End Sub
