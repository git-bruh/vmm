.POSIX:

EXE = vmm
OBJ = vmm.o

all: $(EXE)

vmm: $(OBJ)
	$(CC) $(LDFLAGS) $(OBJ) -o $@

.c.o:
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f *.o $(EXE)
