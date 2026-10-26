' INV: func.split
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/split-function
' EXPECT:  2 
' EXPECT:  1 
Public Sub Main()
    Dim a() As String
    a = Split("aXbxc", "x", -1, vbTextCompare)
    Debug.Print UBound(a)
    a = Split("aXbxc", "x", -1, vbBinaryCompare)
    Debug.Print UBound(a)
End Sub
