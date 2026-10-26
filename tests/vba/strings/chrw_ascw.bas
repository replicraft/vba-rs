' INV: func.chr, func.asc
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: True
' EXPECT: 8364
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(ChrW(8364) = ChrW(&H20AC))
    Debug.Print CStr(AscW(ChrW(8364)))
    Debug.Print CStr(Len(ChrW(8364)))
End Sub
