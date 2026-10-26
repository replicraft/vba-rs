' INV: type.string, type.byte
' SOURCE: derived assigning a Byte array to a String reads the bytes as UTF-16LE code units (the layout of b = s); assigning an empty string gives an empty array
' EXPECT: ab|3
' EXPECT: -1 
Public Sub Main()
    Dim b() As Byte, s As String, t As String
    s = "ab"
    b = s
    t = b
    Debug.Print t & "|" & UBound(b)
    b = ""
    Debug.Print UBound(b)
End Sub
