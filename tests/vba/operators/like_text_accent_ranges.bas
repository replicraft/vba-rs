' INV: op.like
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator with Option Compare Text, [A-E] matches A, a, B, b, E, e and also the accented A (accented letters sort right after their base letter), but not the accented E, which sorts after E
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: False
' EXPECT: False
' EXPECT: False
' EXPECT: True
' EXPECT: True
Option Compare Text
Public Sub Main()
    Debug.Print CStr(ChrW(&HC0) Like "[A-E]")
    Debug.Print CStr(ChrW(&HE0) Like "[A-E]")
    Debug.Print CStr(ChrW(&HE9) Like "[A-D]")
    Debug.Print CStr(ChrW(&HCA) Like "[A-E]")
    Debug.Print CStr(ChrW(&HEA) Like "[A-E]")
    Debug.Print CStr(ChrW(&HC0) Like "[!A-E]")
    Debug.Print CStr(ChrW(&HC0) Like "[!B-E]")
    Debug.Print CStr("xay" Like "x[A-C]y")
End Sub
