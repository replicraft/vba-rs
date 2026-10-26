' INV: stmt.enum
' SOURCE: spec ms-vbal 5.2.3.4 Enum member values
' EXPECT:  0  5  6 
' EXPECT:  6 
Enum Color
    Red
    Green = 5
    Blue
End Enum
Public Sub Main()
    Debug.Print Red; Green; Blue
    Dim c As Color
    c = Blue
    Debug.Print c
End Sub
