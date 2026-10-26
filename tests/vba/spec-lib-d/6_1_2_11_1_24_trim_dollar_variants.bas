' INV: vbal.6.1.2.11.1.24
' SOURCE: derived ms-vbal 6.1.2.11.1.24 LTrim$, RTrim$ and Trim$ remove spaces only, not tabs
' EXPECT: l=[a b  ]
' EXPECT: r=[  a b]
' EXPECT: t=[a b]
' EXPECT: tab=2
' EXPECT: all=0
Public Sub Main()
    Dim s As String
    s = "  a b  "
    Debug.Print "l=[" & LTrim$(s) & "]"
    Debug.Print "r=[" & RTrim$(s) & "]"
    Debug.Print "t=[" & Trim$(s) & "]"
    Debug.Print "tab=" & CStr(Len(Trim$(vbTab & "a")))
    Debug.Print "all=" & CStr(Len(Trim$("    ")))
End Sub
