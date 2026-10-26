' INV: func.strconv, vbal.6.1.2.11.1.37
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/strconv-function vbLinguisticCasing (1024, vbLinguisticCasing) uses linguistic rather than file system casing rules and is valid with vbUpperCase and vbLowerCase only; for the en-US locale the result equals the plain case conversion
' EXPECT: HELLO STRASSE
' EXPECT: hello
' EXPECT: 5
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(StrConv("hello strasse", vbUpperCase + 1024))
    Debug.Print CStr(StrConv("HELLO", vbLowerCase Or 1024))
    Dim v As Variant
    On Error Resume Next
    v = StrConv("abc", 1024)
    Debug.Print CStr(Err.Number)
    Err.Clear
    v = StrConv("abc", vbProperCase + 1024)
    Debug.Print CStr(Err.Number)
End Sub
