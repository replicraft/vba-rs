' INV: func.strcomp
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' EXPECT: True
' EXPECT:  0 
Public Sub Main()
    Debug.Print StrComp(ChrW(233), "e", vbTextCompare) <> 0
    Debug.Print StrComp("a", "A", vbTextCompare)
End Sub
