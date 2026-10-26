' INV: func.asc, func.chr, func.len, func.ucase
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 233
' EXPECT: 233
' EXPECT: 2
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Asc(ChrW(233)))
    Debug.Print CStr(Asc(Chr(233)))
    Debug.Print CStr(Len(ChrW(&HD83D) & ChrW(&HDE00)))
    Debug.Print CStr(UCase(ChrW(233)) = ChrW(201))
    Debug.Print CStr(LCase(ChrW(201)) = ChrW(233))
End Sub
