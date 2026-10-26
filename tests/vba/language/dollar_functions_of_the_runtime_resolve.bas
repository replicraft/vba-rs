' INV: func.curdir, func.dir
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/curdir-function Command$, CurDir$ and Dir$ are the String-returning spellings of Command, CurDir and Dir
' GRANT: file-read
' GRANT: file-write
' EXPECT: []
' EXPECT: True
' EXPECT: True
' EXPECT: []
' EXPECT: String String String
Public Sub Main()
    MkDir "/t"
    ChDir "/t"
    Debug.Print "[" & Command$ & "]"
    Debug.Print CurDir$ = CurDir
    Debug.Print CurDir$("C") = CurDir("C")
    Debug.Print "[" & Dir$("/t/none*") & "]"
    Debug.Print TypeName(Command$) & " " & TypeName(CurDir$) & " " & TypeName(Dir$("/t/none*"))
End Sub
