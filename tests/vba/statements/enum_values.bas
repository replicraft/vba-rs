' INV: stmt.enum
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/enum-statement
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
