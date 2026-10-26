' INV: vbal.6.1.2.2
' SOURCE: spec ms-vbal 6.1.2.2 Constants module string constants
' EXPECT: back=8
' EXPECT: cr=13
' EXPECT: crlf=2:13,10
' EXPECT: ff=12
' EXPECT: lf=10
' EXPECT: nul=1:0
' EXPECT: tab=9
' EXPECT: vt=11
' EXPECT: nullstring=0
' EXPECT: objerr=-2147221504
Public Sub Main()
    Debug.Print "back=" & CStr(Asc(vbBack))
    Debug.Print "cr=" & CStr(Asc(vbCr))
    Debug.Print "crlf=" & CStr(Len(vbCrLf)) & ":" & CStr(Asc(Left$(vbCrLf, 1))) & "," & CStr(Asc(Right$(vbCrLf, 1)))
    Debug.Print "ff=" & CStr(Asc(vbFormFeed))
    Debug.Print "lf=" & CStr(Asc(vbLf))
    Debug.Print "nul=" & CStr(Len(vbNullChar)) & ":" & CStr(Asc(vbNullChar))
    Debug.Print "tab=" & CStr(Asc(vbTab))
    Debug.Print "vt=" & CStr(Asc(vbVerticalTab))
    Debug.Print "nullstring=" & CStr(Len(vbNullString))
    Debug.Print "objerr=" & CStr(vbObjectError)
End Sub
