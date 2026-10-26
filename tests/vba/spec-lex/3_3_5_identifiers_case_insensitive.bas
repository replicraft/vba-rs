' INV: vbal.3.3.5, lex.case-insensitive
' SOURCE: spec ms-vbal 3.3.5 identifier tokens, upper and lower case Latin letters are equivalent in identifiers
' EXPECT: 5
' EXPECT: 6
' EXPECT: 7
Public Sub Main()
    Dim MyVar As Long
    myvar = 5
    Debug.Print CStr(MYVAR)
    DOIT
    dim AbC_1 as long
    abc_1 = 7
    Debug.Print CStr(ABC_1)
End Sub

Private Sub DoIt()
    Debug.Print "6"
End Sub
