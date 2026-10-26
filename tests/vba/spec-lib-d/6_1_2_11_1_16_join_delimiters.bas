' INV: vbal.6.1.2.11.1.16, func.join
' SOURCE: spec ms-vbal 6.1.2.11.1.16 Join delimiter default space, zero-length delimiter concatenates
' EXPECT: a=[a,b,c]
' EXPECT: b=[a b c]
' EXPECT: c=[abc]
' EXPECT: d=[a--b--c]
' EXPECT: e=[]
' EXPECT: t=String
Public Sub Main()
    Dim v As Variant
    v = Array("a", "b", "c")
    Debug.Print "a=[" & Join(v, ",") & "]"
    Debug.Print "b=[" & Join(v) & "]"
    Debug.Print "c=[" & Join(v, "") & "]"
    Debug.Print "d=[" & Join(v, "--") & "]"
    Debug.Print "e=[" & Join(Array(), ",") & "]"
    Debug.Print "t=" & TypeName(Join(v, ","))
End Sub
