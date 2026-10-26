' INV: lit.hex, lit.octal
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/hex-function
' EXPECT:  255  32767  15  16 
' EXPECT: -1  65535 
Public Sub Main()
    Debug.Print &HFF; &H7FFF; &O17; &H10&
    Debug.Print &HFFFF; &HFFFF&
End Sub
