' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 a member of an Enum classifies as a value; it may be qualified by the Enum name
' METHOD: by hand: Red = 1, Blue = 3
' EXPECT:  1 
' EXPECT:  3 
' EXPECT:  4 
Public Enum Color
    Red = 1
    Green
    Blue
End Enum
Public Sub Main()
    Debug.Print Color.Red
    Debug.Print Blue
    Debug.Print Red + Blue
End Sub
