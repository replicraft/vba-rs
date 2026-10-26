' INV: func.str
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/str-function
' EXPECT: [ 459]
' EXPECT: [-459.65]
' EXPECT: [ 459.001]
Public Sub Main()
    Debug.Print "[" & Str(459) & "]"
    Debug.Print "[" & Str(-459.65) & "]"
    Debug.Print "[" & Str(459.001) & "]"
End Sub
