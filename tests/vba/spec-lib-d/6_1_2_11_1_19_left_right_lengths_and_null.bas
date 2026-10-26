' INV: vbal.6.1.2.11.1.19, func.left, func.right
' SOURCE: spec ms-vbal 6.1.2.11.1.19 Left/Right length 0, length >= Len, Null
' EXPECT: a=[ab]
' EXPECT: b=[]
' EXPECT: c=[abc]
' EXPECT: d=[abc]
' EXPECT: e=[bc]
' EXPECT: f=[]
' EXPECT: g=[abc]
' EXPECT: h=[abc]
' EXPECT: ln=True
' EXPECT: rn=True
' EXPECT: lt=String
Public Sub Main()
    Debug.Print "a=[" & Left("abc", 2) & "]"
    Debug.Print "b=[" & Left("abc", 0) & "]"
    Debug.Print "c=[" & Left("abc", 3) & "]"
    Debug.Print "d=[" & Left("abc", 99) & "]"
    Debug.Print "e=[" & Right("abc", 2) & "]"
    Debug.Print "f=[" & Right("abc", 0) & "]"
    Debug.Print "g=[" & Right("abc", 3) & "]"
    Debug.Print "h=[" & Right("abc", 99) & "]"
    Debug.Print "ln=" & CStr(IsNull(Left(Null, 2)))
    Debug.Print "rn=" & CStr(IsNull(Right(Null, 2)))
    Debug.Print "lt=" & TypeName(Left$("abc", 1))
End Sub
