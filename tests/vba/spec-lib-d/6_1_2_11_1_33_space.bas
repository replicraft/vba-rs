' INV: vbal.6.1.2.11.1.33, func.space
' SOURCE: spec ms-vbal 6.1.2.11.1.33 Space returns a string of the given number of spaces
' EXPECT: a=3
' EXPECT: b=0
' EXPECT: c=True
' EXPECT: t1=String
' EXPECT: t2=String
' EXPECT: d=[   ]
Public Sub Main()
    Debug.Print "a=" & CStr(Len(Space(3)))
    Debug.Print "b=" & CStr(Len(Space(0)))
    Debug.Print "c=" & CStr(Space(2) = "  ")
    Debug.Print "t1=" & TypeName(Space(2))
    Debug.Print "t2=" & TypeName(Space$(2))
    Debug.Print "d=[" & Space$(3) & "]"
End Sub
