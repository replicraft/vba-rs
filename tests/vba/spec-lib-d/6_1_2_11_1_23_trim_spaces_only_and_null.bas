' INV: vbal.6.1.2.11.1.23, func.trim, func.ltrim, func.rtrim
' SOURCE: spec ms-vbal 6.1.2.11.1.23 LTrim/RTrim/Trim remove spaces only; Null gives Null
' EXPECT: l=[a b  ]
' EXPECT: r=[  a b]
' EXPECT: t=[a b]
' EXPECT: tab=2
' EXPECT: nl=True
' EXPECT: nr=True
' EXPECT: nt=True
' EXPECT: s=[a]
Public Sub Main()
    Debug.Print "l=[" & LTrim("  a b  ") & "]"
    Debug.Print "r=[" & RTrim("  a b  ") & "]"
    Debug.Print "t=[" & Trim("  a b  ") & "]"
    Debug.Print "tab=" & CStr(Len(Trim(vbTab & "a")) )
    Debug.Print "nl=" & CStr(IsNull(LTrim(Null)))
    Debug.Print "nr=" & CStr(IsNull(RTrim(Null)))
    Debug.Print "nt=" & CStr(IsNull(Trim(Null)))
    Debug.Print "s=[" & Trim$("  a  ") & "]"
End Sub
