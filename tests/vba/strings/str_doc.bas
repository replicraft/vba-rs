' INV: func.str
' SOURCE: spec ms-vbal 6.1.2.3.1.20 Str leading sign space and 5.5.1.2.4 number to String
' EXPECT: [ 459]
' EXPECT: [-459.65]
' EXPECT: [ 459.001]
Public Sub Main()
    Debug.Print "[" & Str(459) & "]"
    Debug.Print "[" & Str(-459.65) & "]"
    Debug.Print "[" & Str(459.001) & "]"
End Sub
