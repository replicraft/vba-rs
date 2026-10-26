' INV: vbal.6.1.2.11.1.29, func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace start (result begins at Start), count, compare
' EXPECT: all=[xxxx]
' EXPECT: start=[caxc]
' EXPECT: count=[bbaa]
' EXPECT: bin=[aAa]
' EXPECT: txt=[xxx]
' EXPECT: grow=[aaa]
Public Sub Main()
    Debug.Print "all=[" & Replace("aaaa", "a", "x") & "]"
    Debug.Print "start=[" & Replace("abcabc", "b", "x", 3) & "]"
    Debug.Print "count=[" & Replace("aaaa", "a", "b", 1, 2) & "]"
    Debug.Print "bin=[" & Replace("aAa", "x", "y", 1, -1, vbBinaryCompare) & "]"
    Debug.Print "txt=[" & Replace("aAa", "a", "x", 1, -1, vbTextCompare) & "]"
    Debug.Print "grow=[" & Replace("a", "a", "aaa") & "]"
End Sub
