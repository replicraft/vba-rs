' INV: func.split
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/split-function
' EXPECT:  8  0 
' EXPECT: [Dow ][ Fonseca ]
' EXPECT: [Dow][Fonseca]
' EXPECT: Kopke
' EXPECT: Kopke
Public Sub Main()
    Dim full As String, a() As String, b() As String
    full = "Dow - Fonseca - Graham - Kopke - Noval - Offley - Sandeman - Taylor - Warre"
    a = Split(full, "-")
    b = Split(full, " - ")
    Debug.Print UBound(a); LBound(a)
    Debug.Print "[" & a(0) & "][" & a(1) & "]"
    Debug.Print "[" & b(0) & "][" & b(1) & "]"
    Debug.Print b(3)
    Debug.Print Split(full, " - ")(3)
End Sub
