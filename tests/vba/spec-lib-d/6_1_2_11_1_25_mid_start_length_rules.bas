' INV: vbal.6.1.2.11.1.25, func.mid
' SOURCE: spec ms-vbal 6.1.2.11.1.25 Mid start beyond length, omitted or oversized Length, Null
' EXPECT: a=[bcd]
' EXPECT: b=[bc]
' EXPECT: c=[]
' EXPECT: d=[cde]
' EXPECT: e=[de]
' EXPECT: f=[]
' EXPECT: g=True
' EXPECT: h=[bc]
Public Sub Main()
    Debug.Print "a=[" & Mid("abcde", 2, 3) & "]"
    Debug.Print "b=[" & Mid("abcde", 2, 2) & "]"
    Debug.Print "c=[" & Mid("abcde", 6) & "]"
    Debug.Print "d=[" & Mid("abcde", 3) & "]"
    Debug.Print "e=[" & Mid("abcde", 4, 99) & "]"
    Debug.Print "f=[" & Mid("abcde", 20, 2) & "]"
    Debug.Print "g=" & CStr(IsNull(Mid(Null, 1, 2)))
    Debug.Print "h=[" & Mid$("abcde", 2, 2) & "]"
End Sub
